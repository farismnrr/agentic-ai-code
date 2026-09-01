use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

pub const KEY_BYTES: usize = 32;

pub fn random_key() -> Result<[u8; KEY_BYTES], ()> {
    let mut key = [0u8; KEY_BYTES];
    SystemRandom::new().fill(&mut key).map_err(|_| ())?;
    Ok(key)
}

pub fn seal(key: &[u8; KEY_BYTES], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, ()> {
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| ())?;
    let key = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    SystemRandom::new().fill(&mut nonce_bytes).map_err(|_| ())?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut output = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(aad), &mut output)
        .map_err(|_| ())?;
    let mut envelope = nonce_bytes.to_vec();
    envelope.extend_from_slice(&output);
    Ok(envelope)
}

pub fn open(key: &[u8; KEY_BYTES], envelope: &[u8], aad: &[u8]) -> Result<Vec<u8>, ()> {
    if envelope.len() < 12 + aead::MAX_TAG_LEN {
        return Err(());
    }
    let nonce = Nonce::try_assume_unique_for_key(&envelope[..12]).map_err(|_| ())?;
    let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| ())?;
    let key = LessSafeKey::new(unbound);
    let mut ciphertext = envelope[12..].to_vec();
    let plaintext = key
        .open_in_place(nonce, Aad::from(aad), &mut ciphertext)
        .map_err(|_| ())?;
    Ok(plaintext.to_vec())
}
