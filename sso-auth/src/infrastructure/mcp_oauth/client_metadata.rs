use std::time::Duration;

use async_trait::async_trait;
use reqwest::{
    header::{ACCEPT, USER_AGENT},
    redirect::Policy,
};
use serde::Deserialize;
use url::Url;

use crate::application::{
    AuthError, McpClientMetadata, McpClientMetadataResolver,
};

const MAX_METADATA_BYTES: usize = 64 * 1024;

pub struct HttpMcpClientMetadataResolver {
    client: reqwest::Client,
}

impl HttpMcpClientMetadataResolver {
    pub fn new() -> Result<Self, AuthError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(Policy::none())
            .build()
            .map_err(|_| {
                AuthError::InvalidConfiguration("failed to build MCP client metadata HTTP client")
            })?;
        Ok(Self { client })
    }
}

#[async_trait]
impl McpClientMetadataResolver for HttpMcpClientMetadataResolver {
    async fn resolve(&self, client_id: &str) -> Result<McpClientMetadata, AuthError> {
        let url = Url::parse(client_id).map_err(|_| AuthError::InvalidOAuthRequest)?;
        if !trusted_chatgpt_client_id(&url) {
            return Err(AuthError::InvalidOAuthRequest);
        }

        let response = self
            .client
            .get(url)
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, "Masih-Awam-SSO/0.1")
            .send()
            .await
            .map_err(|_| AuthError::ExternalProvider)?
            .error_for_status()
            .map_err(|_| AuthError::ExternalProvider)?;

        if response
            .content_length()
            .is_some_and(|size| size > MAX_METADATA_BYTES as u64)
        {
            return Err(AuthError::InvalidOAuthRequest);
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|_| AuthError::ExternalProvider)?;
        if bytes.len() > MAX_METADATA_BYTES {
            return Err(AuthError::InvalidOAuthRequest);
        }

        let remote: RemoteClientMetadata =
            serde_json::from_slice(&bytes).map_err(|_| AuthError::InvalidOAuthRequest)?;
        let mut methods = remote.token_endpoint_auth_methods_supported;
        if let Some(method) = remote.token_endpoint_auth_method {
            if !methods.iter().any(|candidate| candidate == &method) {
                methods.push(method);
            }
        }

        Ok(McpClientMetadata {
            client_id: remote.client_id,
            redirect_uris: remote.redirect_uris,
            token_endpoint_auth_methods_supported: methods,
        })
    }
}

#[derive(Deserialize)]
struct RemoteClientMetadata {
    client_id: String,
    redirect_uris: Vec<String>,
    #[serde(default)]
    token_endpoint_auth_methods_supported: Vec<String>,
    token_endpoint_auth_method: Option<String>,
}

fn trusted_chatgpt_client_id(url: &Url) -> bool {
    let path = url.path();
    url.scheme() == "https"
        && url.host_str() == Some("chatgpt.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && path.starts_with("/oauth/")
        && path.ends_with("/client.json")
        && path.len() > "/oauth/client.json".len() - 1
}

#[cfg(test)]
mod tests {
    use super::trusted_chatgpt_client_id;
    use url::Url;

    #[test]
    fn trusts_stable_and_callback_specific_chatgpt_cimd_urls() {
        for value in [
            "https://chatgpt.com/oauth/client.json",
            "https://chatgpt.com/oauth/callback-123/client.json",
        ] {
            assert!(trusted_chatgpt_client_id(&Url::parse(value).unwrap()));
        }
    }

    #[test]
    fn rejects_non_chatgpt_or_redirectable_client_ids() {
        for value in [
            "https://example.com/oauth/client.json",
            "https://chatgpt.com/",
            "https://chatgpt.com/oauth/client.json?next=x",
            "http://chatgpt.com/oauth/client.json",
        ] {
            assert!(!trusted_chatgpt_client_id(&Url::parse(value).unwrap()));
        }
    }
}
