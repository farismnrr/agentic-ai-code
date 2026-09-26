use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use crate::{
    application::{AuthError, McpAccessTokenIssuer},
    domain::AuthenticatedUser,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize)]
struct AccessTokenClaims<'a> {
    typ: &'static str,
    iss: &'a str,
    aud: &'a str,
    sub: String,
    login: &'a str,
    avatar_url: &'a Option<String>,
    scope: &'a str,
    iat: u64,
    exp: u64,
}

pub struct SignedMcpAccessTokenIssuer {
    secret: Vec<u8>,
    issuer: String,
}

impl SignedMcpAccessTokenIssuer {
    pub fn new(secret: String, issuer: String) -> Result<Self, AuthError> {
        if secret.len() < 32 {
            return Err(AuthError::InvalidConfiguration(
                "RELAY_ASSERTION_SECRET must contain at least 32 bytes",
            ));
        }
        Ok(Self {
            secret: secret.into_bytes(),
            issuer,
        })
    }
}

impl McpAccessTokenIssuer for SignedMcpAccessTokenIssuer {
    fn issue(
        &self,
        user: &AuthenticatedUser,
        audience: &str,
        scope: &str,
        ttl_seconds: u64,
    ) -> Result<String, AuthError> {
        let issued_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuthError::InvalidOAuthRequest)?
            .as_secs();
        let claims = AccessTokenClaims {
            typ: "mcp_access",
            iss: &self.issuer,
            aud: audience,
            sub: format!("github:{}", user.id),
            login: &user.login,
            avatar_url: &user.avatar_url,
            scope,
            iat: issued_at,
            exp: issued_at.saturating_add(ttl_seconds),
        };
        let payload = serde_json::to_vec(&claims).map_err(|_| AuthError::InvalidOAuthRequest)?;
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| AuthError::InvalidOAuthRequest)?;
        mac.update(&payload);

        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        ))
    }
}
