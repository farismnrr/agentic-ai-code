use async_trait::async_trait;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use url::Url;

use crate::{
    application::{AuthError, OAuthProvider},
    domain::AuthenticatedUser,
};

pub struct GitHubOAuthClient {
    client: reqwest::Client,
    client_id: String,
    client_secret: String,
    callback_url: String,
    authorize_url: Url,
    token_url: Url,
    user_url: Url,
}

impl GitHubOAuthClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        callback_url: String,
        authorize_url: String,
        token_url: String,
        api_url: String,
    ) -> Result<Self, AuthError> {
        let authorize_url = Url::parse(&authorize_url).map_err(|_| {
            AuthError::InvalidConfiguration("GITHUB_AUTHORIZE_URL must be a valid URL")
        })?;
        let token_url = Url::parse(&token_url)
            .map_err(|_| AuthError::InvalidConfiguration("GITHUB_TOKEN_URL must be a valid URL"))?;
        let api_url = Url::parse(&api_url)
            .map_err(|_| AuthError::InvalidConfiguration("GITHUB_API_URL must be a valid URL"))?;
        let user_url = api_url.join("user").map_err(|_| {
            AuthError::InvalidConfiguration("GITHUB_API_URL must support relative paths")
        })?;

        Ok(Self {
            client: reqwest::Client::new(),
            client_id,
            client_secret,
            callback_url,
            authorize_url,
            token_url,
            user_url,
        })
    }
}

#[async_trait]
impl OAuthProvider for GitHubOAuthClient {
    fn authorization_url(&self, state: &str) -> String {
        let mut url = self.authorize_url.clone();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.callback_url)
            .append_pair("state", state)
            .append_pair("scope", "read:user");
        url.into()
    }

    async fn authenticate(&self, code: &str) -> Result<AuthenticatedUser, AuthError> {
        let token = self
            .client
            .post(self.token_url.clone())
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, "Masih-Awam-SSO")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", self.callback_url.as_str()),
            ])
            .send()
            .await
            .map_err(|_| AuthError::ExternalProvider)?
            .error_for_status()
            .map_err(|_| AuthError::ExternalProvider)?
            .json::<TokenResponse>()
            .await
            .map_err(|_| AuthError::ExternalProvider)?;

        let access_token = token.access_token.ok_or(AuthError::ExternalProvider)?;
        let user = self
            .client
            .get(self.user_url.clone())
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "Masih-Awam-SSO")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|_| AuthError::ExternalProvider)?
            .error_for_status()
            .map_err(|_| AuthError::ExternalProvider)?
            .json::<GitHubUser>()
            .await
            .map_err(|_| AuthError::ExternalProvider)?;

        Ok(AuthenticatedUser {
            id: user.id,
            login: user.login,
            avatar_url: user.avatar_url,
        })
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
    avatar_url: Option<String>,
}
