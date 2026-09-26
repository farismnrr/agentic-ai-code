use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use crate::{
    application::{AuthError, ConnectionAssertionIssuer},
    domain::AuthenticatedUser,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize)]
struct AssertionClaims<'a> {
    iss: &'a str,
    aud: &'a str,
    sub: String,
    login: &'a str,
    avatar_url: &'a Option<String>,
    state: &'a str,
    iat: u64,
    exp: u64,
}

pub struct SignedRelayAssertionIssuer {
    secret: Vec<u8>,
    issuer: String,
    audience: String,
    ttl_seconds: u64,
}

impl SignedRelayAssertionIssuer {
    pub fn new(
        secret: String,
        issuer: String,
        audience: String,
        ttl_seconds: u64,
    ) -> Result<Self, AuthError> {
        if secret.len() < 32 {
            return Err(AuthError::InvalidConfiguration(
                "RELAY_ASSERTION_SECRET must contain at least 32 bytes",
            ));
        }
        if ttl_seconds == 0 || ttl_seconds > 300 {
            return Err(AuthError::InvalidConfiguration(
                "RELAY_ASSERTION_TTL_SECONDS must be between 1 and 300",
            ));
        }

        Ok(Self {
            secret: secret.into_bytes(),
            issuer,
            audience,
            ttl_seconds,
        })
    }

    fn now() -> Result<u64, AuthError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|_| AuthError::InvalidConnectionAssertion)
    }
}

impl ConnectionAssertionIssuer for SignedRelayAssertionIssuer {
    fn issue(&self, user: &AuthenticatedUser, state: &str) -> Result<String, AuthError> {
        let issued_at = Self::now()?;
        let claims = AssertionClaims {
            iss: &self.issuer,
            aud: &self.audience,
            sub: format!("github:{}", user.id),
            login: &user.login,
            avatar_url: &user.avatar_url,
            state,
            iat: issued_at,
            exp: issued_at.saturating_add(self.ttl_seconds),
        };
        let payload =
            serde_json::to_vec(&claims).map_err(|_| AuthError::InvalidConnectionAssertion)?;
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| AuthError::InvalidConnectionAssertion)?;
        mac.update(&payload);
        let signature = mac.finalize().into_bytes();

        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }
}
