use url::{ParseError, Url};

use crate::application::SsoConnectUrlProvider;

pub struct SsoConnectUrlBuilder {
    login_url: Url,
}

impl SsoConnectUrlBuilder {
    pub fn new(sso_base_url: Url) -> Result<Self, ParseError> {
        Ok(Self {
            login_url: sso_base_url.join("/auth/github")?,
        })
    }
}

impl SsoConnectUrlProvider for SsoConnectUrlBuilder {
    fn connect_url(&self, state: &str) -> String {
        let mut url = self.login_url.clone();
        url.query_pairs_mut().append_pair("connection_state", state);
        url.to_string()
    }
}
