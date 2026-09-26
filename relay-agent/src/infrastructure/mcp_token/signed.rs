use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    application::{AuthError, McpAccessTokenVerifier},
    domain::VerifiedPrincipal,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
struct AccessTokenClaims {
    typ: String,
    iss: String,
    aud: String,
    sub: String,
    login: String,
    avatar_url: Option<String>,
    scope: String,
    iat: u64,
    exp: u64,
}

pub struct SignedMcpAccessTokenVerifier {
    secret: Vec<u8>,
    expected_issuer: String,
    expected_audience: String,
}

impl SignedMcpAccessTokenVerifier {
    pub fn new(
        secret: String,
        expected_issuer: String,
        expected_audience: String,
    ) -> Result<Self, AuthError> {
        if secret.len() < 32 {
            return Err(AuthError::InvalidAccessToken);
        }
        Ok(Self {
            secret: secret.into_bytes(),
            expected_issuer,
            expected_audience,
        })
    }
}

impl McpAccessTokenVerifier for SignedMcpAccessTokenVerifier {
    fn verify(&self, token: &str, required_scope: &str) -> Result<VerifiedPrincipal, AuthError> {
        let (payload, signature) = token
            .split_once('.')
            .ok_or(AuthError::InvalidAccessToken)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| AuthError::InvalidAccessToken)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| AuthError::InvalidAccessToken)?;

        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| AuthError::InvalidAccessToken)?;
        mac.update(&payload);
        mac.verify_slice(&signature)
            .map_err(|_| AuthError::InvalidAccessToken)?;

        let claims: AccessTokenClaims =
            serde_json::from_slice(&payload).map_err(|_| AuthError::InvalidAccessToken)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuthError::InvalidAccessToken)?
            .as_secs();
        let valid = claims.typ == "mcp_access"
            && claims.iss == self.expected_issuer
            && claims.aud == self.expected_audience
            && !claims.sub.is_empty()
            && !claims.login.is_empty()
            && claims
                .scope
                .split_whitespace()
                .any(|scope| scope == required_scope)
            && claims.iat <= now.saturating_add(30)
            && claims.exp > now
            && claims.exp > claims.iat;
        if !valid {
            return Err(AuthError::InvalidAccessToken);
        }

        Ok(VerifiedPrincipal::new(
            claims.sub,
            claims.login,
            claims.avatar_url,
        ))
    }
}
