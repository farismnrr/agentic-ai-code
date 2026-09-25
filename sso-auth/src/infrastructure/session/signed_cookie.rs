use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{
    application::{AuthError, SessionCodec},
    domain::{AuthenticatedUser, AuthSession},
};

type HmacSha256 = Hmac<Sha256>;

pub struct SignedSessionCodec {
    secret: Vec<u8>,
    ttl_seconds: u64,
}

impl SignedSessionCodec {
    pub fn new(secret: String, ttl_seconds: u64) -> Result<Self, AuthError> {
        if secret.as_bytes().len() < 32 {
            return Err(AuthError::InvalidConfiguration("SESSION_SECRET must contain at least 32 bytes"));
        }
        Ok(Self { secret: secret.into_bytes(), ttl_seconds })
    }

    fn now() -> Result<u64, AuthError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .map_err(|_| AuthError::InvalidSession)
    }
}

impl SessionCodec for SignedSessionCodec {
    fn issue(&self, user: AuthenticatedUser) -> Result<(String, AuthSession), AuthError> {
        let issued_at = Self::now()?;
        let session = AuthSession {
            user,
            issued_at,
            expires_at: issued_at.saturating_add(self.ttl_seconds),
        };
        let payload = serde_json::to_vec(&session).map_err(|_| AuthError::InvalidSession)?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).map_err(|_| AuthError::InvalidSession)?;
        mac.update(&payload);
        let signature = mac.finalize().into_bytes();

        Ok((
            format!("{}.{}", URL_SAFE_NO_PAD.encode(payload), URL_SAFE_NO_PAD.encode(signature)),
            session,
        ))
    }

    fn decode(&self, token: &str) -> Result<AuthSession, AuthError> {
        let (payload, signature) = token.split_once('.').ok_or(AuthError::InvalidSession)?;
        let payload = URL_SAFE_NO_PAD.decode(payload).map_err(|_| AuthError::InvalidSession)?;
        let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| AuthError::InvalidSession)?;

        let mut mac = HmacSha256::new_from_slice(&self.secret).map_err(|_| AuthError::InvalidSession)?;
        mac.update(&payload);
        mac.verify_slice(&signature).map_err(|_| AuthError::InvalidSession)?;

        let session: AuthSession = serde_json::from_slice(&payload).map_err(|_| AuthError::InvalidSession)?;
        if session.expires_at <= Self::now()? {
            return Err(AuthError::InvalidSession);
        }
        Ok(session)
    }
}
