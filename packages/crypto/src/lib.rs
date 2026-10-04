use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroizing;

const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("key must be exactly 32 bytes")]
    InvalidKey,
    #[error("key or payload is not valid base64")]
    InvalidEncoding,
    #[error("encrypted payload is incomplete")]
    IncompletePayload,
    #[error("authenticated encryption failed")]
    EncryptionFailed,
    #[error("decrypted data is not valid UTF-8")]
    InvalidUtf8,
}

#[derive(Clone)]
pub struct EncryptionKey(Zeroizing<[u8; KEY_SIZE]>);

impl EncryptionKey {
    pub fn generate() -> Self {
        let mut bytes = [0_u8; KEY_SIZE];
        OsRng.fill_bytes(&mut bytes);
        Self(Zeroizing::new(bytes))
    }

    pub fn from_base64(encoded: &str) -> Result<Self, CryptoError> {
        let bytes = BASE64
            .decode(encoded)
            .map_err(|_| CryptoError::InvalidEncoding)?;
        let key = bytes.try_into().map_err(|_| CryptoError::InvalidKey)?;
        Ok(Self(Zeroizing::new(key)))
    }

    pub fn to_base64(&self) -> String {
        BASE64.encode(self.0.as_slice())
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.0.as_slice()))
    }
}

pub fn encrypt(key: &EncryptionKey, plaintext: &[u8]) -> Result<String, CryptoError> {
    let mut nonce_bytes = [0_u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = key
        .cipher()
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let mut payload = nonce_bytes.to_vec();
    payload.extend(ciphertext);
    Ok(BASE64.encode(payload))
}

pub fn decrypt(key: &EncryptionKey, encoded: &str) -> Result<Vec<u8>, CryptoError> {
    let payload = BASE64
        .decode(encoded)
        .map_err(|_| CryptoError::InvalidEncoding)?;
    if payload.len() <= NONCE_SIZE {
        return Err(CryptoError::IncompletePayload);
    }
    key.cipher()
        .decrypt(
            Nonce::from_slice(&payload[..NONCE_SIZE]),
            &payload[NONCE_SIZE..],
        )
        .map_err(|_| CryptoError::EncryptionFailed)
}

pub fn decrypt_text(key: &EncryptionKey, encoded: &str) -> Result<String, CryptoError> {
    String::from_utf8(decrypt(key, encoded)?).map_err(|_| CryptoError::InvalidUtf8)
}

pub fn fingerprint(value: &[u8]) -> String {
    Sha256::digest(value)
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_tamper_detection() {
        let key = EncryptionKey::generate();
        let encrypted = encrypt(&key, b"private message").expect("encryption");
        assert_eq!(
            decrypt_text(&key, &encrypted).expect("decryption"),
            "private message"
        );
        let mut tampered = BASE64.decode(encrypted).expect("payload");
        tampered[NONCE_SIZE] ^= 1;
        assert!(decrypt(&key, &BASE64.encode(tampered)).is_err());
    }

    #[test]
    fn rejects_invalid_key() {
        assert!(matches!(
            EncryptionKey::from_base64("aW52YWxpZA=="),
            Err(CryptoError::InvalidKey)
        ));
    }
}
