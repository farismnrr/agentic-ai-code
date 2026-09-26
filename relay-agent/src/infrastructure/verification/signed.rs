use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    application::{AuthError, SignedAssertionVerifier},
    domain::{VerifiedConnectionAssertion, VerifiedPrincipal},
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
struct AssertionClaims {
    iss: String,
    aud: String,
    sub: String,
    login: String,
    avatar_url: Option<String>,
    state: String,
    iat: u64,
    exp: u64,
}

pub struct SignedRelayAssertionVerifier {
    secret: Vec<u8>,
    expected_issuer: String,
    expected_audience: String,
}

impl SignedRelayAssertionVerifier {
    pub fn new(
        secret: String,
        expected_issuer: String,
        expected_audience: String,
    ) -> Result<Self, AuthError> {
        if secret.len() < 32 {
            return Err(AuthError::InvalidAssertion);
        }
        Ok(Self {
            secret: secret.into_bytes(),
            expected_issuer,
            expected_audience,
        })
    }

    fn now() -> Result<u64, AuthError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|_| AuthError::InvalidAssertion)
    }
}

impl SignedAssertionVerifier for SignedRelayAssertionVerifier {
    fn verify(&self, assertion: &str) -> Result<VerifiedConnectionAssertion, AuthError> {
        let (payload, signature) = assertion
            .split_once('.')
            .ok_or(AuthError::InvalidAssertion)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| AuthError::InvalidAssertion)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| AuthError::InvalidAssertion)?;

        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| AuthError::InvalidAssertion)?;
        mac.update(&payload);
        mac.verify_slice(&signature)
            .map_err(|_| AuthError::InvalidAssertion)?;

        let claims: AssertionClaims =
            serde_json::from_slice(&payload).map_err(|_| AuthError::InvalidAssertion)?;
        let now = Self::now()?;
        let valid = claims.iss == self.expected_issuer
            && claims.aud == self.expected_audience
            && !claims.sub.is_empty()
            && !claims.login.is_empty()
            && !claims.state.is_empty()
            && claims.iat <= now.saturating_add(30)
            && claims.exp > now
            && claims.exp > claims.iat;
        if !valid {
            return Err(AuthError::InvalidAssertion);
        }

        Ok(VerifiedConnectionAssertion {
            principal: VerifiedPrincipal::new(claims.sub, claims.login, claims.avatar_url),
            state: claims.state,
        })
    }
}
