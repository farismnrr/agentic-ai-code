use url::{ParseError, Url};

use crate::application::SsoLoginUrlProvider;

pub struct SsoLoginUrlBuilder {
    login_url: Url,
}

impl SsoLoginUrlBuilder {
    pub fn new(sso_base_url: Url, relay_public_url: Url) -> Result<Self, ParseError> {
        let callback_url = relay_public_url.join("/auth/callback")?;
        let mut login_url = sso_base_url.join("/auth/github")?;
        login_url
            .query_pairs_mut()
            .append_pair("return_to", callback_url.as_str());

        Ok(Self { login_url })
    }
}

impl SsoLoginUrlProvider for SsoLoginUrlBuilder {
    fn login_url(&self) -> String {
        self.login_url.to_string()
    }
}
