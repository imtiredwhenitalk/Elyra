use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

fn decode_key(encoded: &str) -> Result<Key<Aes256Gcm>, String> {
    let bytes = BASE64
        .decode(encoded)
        .map_err(|_| "Encryption key is not valid base64".to_string())?;
    if bytes.len() != KEY_SIZE {
        return Err("Encryption key must contain 32 bytes".to_string());
    }
    Ok(*Key::<Aes256Gcm>::from_slice(&bytes))
}

#[tauri::command]
pub fn generate_encryption_key() -> Result<String, String> {
    let mut bytes = [0_u8; KEY_SIZE];
    OsRng.fill_bytes(&mut bytes);
    Ok(BASE64.encode(bytes))
}

#[tauri::command]
pub fn encrypt_text(key: String, plaintext: String) -> Result<String, String> {
    let cipher = Aes256Gcm::new(&decode_key(&key)?);
    let mut nonce_bytes = [0_u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| "Encryption failed".to_string())?;
    let mut payload = nonce_bytes.to_vec();
    payload.extend(ciphertext);
    Ok(BASE64.encode(payload))
}

#[tauri::command]
pub fn decrypt_text(key: String, payload: String) -> Result<String, String> {
    let cipher = Aes256Gcm::new(&decode_key(&key)?);
    let payload = BASE64
        .decode(payload)
        .map_err(|_| "Encrypted payload is not valid base64".to_string())?;
    if payload.len() <= NONCE_SIZE {
        return Err("Encrypted payload is incomplete".to_string());
    }
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&payload[..NONCE_SIZE]), &payload[NONCE_SIZE..])
        .map_err(|_| "Decryption failed: wrong key or damaged payload".to_string())?;
    String::from_utf8(plaintext).map_err(|_| "Decrypted payload is not UTF-8".to_string())
}

#[tauri::command]
pub fn fingerprint(value: String) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
        .chunks(4)
        .map(|chunk| chunk.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_and_decrypts_with_generated_key() {
        let key = generate_encryption_key().expect("key");
        let encrypted = encrypt_text(key.clone(), "private note".to_string()).expect("encrypt");
        assert_ne!(encrypted, "private note");
        assert_eq!(decrypt_text(key, encrypted).expect("decrypt"), "private note");
    }

    #[test]
    fn rejects_wrong_key() {
        let first = generate_encryption_key().expect("key");
        let second = generate_encryption_key().expect("key");
        let encrypted = encrypt_text(first, "private note".to_string()).expect("encrypt");
        assert!(decrypt_text(second, encrypted).is_err());
    }
}