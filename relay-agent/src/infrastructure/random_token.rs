use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};

use crate::application::TokenGenerator;

pub struct SecureTokenGenerator;

impl TokenGenerator for SecureTokenGenerator {
    fn generate(&self) -> String {
        let mut bytes = [0_u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }
}
