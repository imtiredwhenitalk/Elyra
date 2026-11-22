use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use tauri::AppHandle;

use elyra_crypto::{MessageEnvelope, RecipientPublicKey};

use crate::{secrets, storage};

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
        .decrypt(
            Nonce::from_slice(&payload[..NONCE_SIZE]),
            &payload[NONCE_SIZE..],
        )
        .map_err(|_| "Decryption failed: wrong key or damaged payload".to_string())?;
    String::from_utf8(plaintext).map_err(|_| "Decrypted payload is not UTF-8".to_string())
}

fn ensure_recipient_trust(
    database: &rusqlite::Connection,
    recipient_id: i64,
) -> Result<(), String> {
    let trust_state: Option<String> = database
        .query_row(
            "SELECT trust_state FROM crypto_contacts WHERE id = ?1",
            params![recipient_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match trust_state.as_deref() {
        Some("verified") => Ok(()),
        Some("revoked") => Err("Recipient key is revoked; new encryption is blocked.".to_string()),
        Some("changed") => {
            Err("Recipient key changed; verify the fingerprint before sending.".to_string())
        }
        Some("unverified") => {
            Err("Recipient key is unverified; verify the fingerprint before sending.".to_string())
        }
        Some(other) => Err(format!("Recipient trust state is unknown: {other}")),
        None => Err("Recipient key not found.".to_string()),
    }
}

#[tauri::command]
pub fn encrypt_text_for_recipient(
    app: AppHandle,
    recipient_id: i64,
    key: String,
    plaintext: String,
) -> Result<String, String> {
    let database = storage::connection(&app)?;
    ensure_recipient_trust(&database, recipient_id)?;
    encrypt_text(key, plaintext)
}

fn active_identity_key(database: &rusqlite::Connection) -> Result<(i64, String, String), String> {
    let (id, public_key, fingerprint): (i64, String, String) = database
        .query_row(
            "SELECT id, public_key, fingerprint
             FROM crypto_keys
             WHERE status = 'active'
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No active Ed25519 identity key found.".to_string())?;
    let private_key = secrets::get_secret(&secrets::crypto_key_user(id))
        .map_err(|error| format!("Could not read identity private key: {error}"))?
        .ok_or_else(|| "Active identity private key is missing from the OS keychain.".to_string())?;
    let _ = public_key;
    Ok((id, fingerprint, private_key))
}

fn ensure_sender_trust(
    database: &rusqlite::Connection,
    sender_identity_key_id: &str,
) -> Result<(String, String), String> {
    let (trust_state, public_key): (String, String) = database
        .query_row(
            "SELECT trust_state, public_key
             FROM crypto_contacts
             WHERE fingerprint = ?1",
            params![sender_identity_key_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Sender identity key is not in the contact trust store.".to_string())?;
    match trust_state.as_str() {
        "verified" => Ok((public_key, trust_state)),
        "revoked" => Err("Sender key is revoked; message opening is blocked.".to_string()),
        "changed" => Err("Sender key changed; verify the fingerprint before opening.".to_string()),
        "unverified" => Err("Sender key is unverified; verify the fingerprint before opening.".to_string()),
        other => Err(format!("Sender trust state is unknown: {other}")),
    }
}

#[tauri::command]
pub fn seal_message_envelope(
    app: AppHandle,
    recipient_id: i64,
    plaintext: String,
) -> Result<String, String> {
    let database = storage::connection(&app)?;
    ensure_recipient_trust(&database, recipient_id)?;
    let (recipient_trust_state, recipient_public_key, recipient_encryption_key_id): (
        String,
        Option<String>,
        Option<String>,
    ) = database
        .query_row(
            "SELECT trust_state, encryption_public_key, encryption_fingerprint
             FROM crypto_contacts WHERE id = ?1",
            params![recipient_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Recipient key not found.".to_string())?;
    if recipient_trust_state != "verified" {
        return Err("Recipient key is not verified; verify the fingerprint before sending.".to_string());
    }
    let recipient_public_key =
        recipient_public_key.ok_or_else(|| "Recipient X25519 encryption key is not imported.".to_string())?;
    let recipient_encryption_key_id = recipient_encryption_key_id
        .ok_or_else(|| "Recipient X25519 encryption fingerprint is missing.".to_string())?;
    let (identity_id, sender_fingerprint, sender_private_key) = active_identity_key(&database)?;
    let _ = identity_id;
    let recipients = [RecipientPublicKey {
        key_id: recipient_encryption_key_id,
        public_key: recipient_public_key,
    }];
    let envelope = elyra_crypto::seal_message_envelope(
        &sender_fingerprint,
        &sender_private_key,
        &recipients,
        plaintext.as_bytes(),
    )
    .map_err(|error| format!("Could not seal Elyra message envelope: {error}"))?;
    envelope
        .to_json()
        .map_err(|error| format!("Could not encode Elyra message envelope: {error}"))
}

#[tauri::command]
pub fn open_message_envelope(
    app: AppHandle,
    envelope_json: String,
) -> Result<String, String> {
    let database = storage::connection(&app)?;
    let envelope = MessageEnvelope::from_json(&envelope_json)
        .map_err(|error| format!("Could not decode Elyra message envelope: {error}"))?;
    let (sender_public_key, _) =
        ensure_sender_trust(&database, &envelope.sender_identity_key_id)?;

    let recipient_key_id = envelope
        .recipient_ciphertexts
        .iter()
        .find_map(|recipient| {
            let fingerprint: Option<String> = database
                .query_row(
                    "SELECT fingerprint FROM crypto_encryption_keys
                     WHERE fingerprint = ?1 AND status = 'active'",
                    params![recipient.recipient_key_id],
                    |row| row.get(0),
                )
                .optional()
                .ok()
                .flatten();
            fingerprint.map(|_| recipient.recipient_key_id.clone())
        })
        .ok_or_else(|| "No active local X25519 recipient key matches this envelope.".to_string())?;

    let (recipient_id, recipient_private_key): (i64, String) = database
        .query_row(
            "SELECT id, public_key FROM crypto_encryption_keys
             WHERE fingerprint = ?1 AND status = 'active'",
            params![recipient_key_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    let private_key = secrets::get_secret(&secrets::crypto_encryption_key_user(recipient_id))
        .map_err(|error| format!("Could not read recipient private key: {error}"))?
        .ok_or_else(|| "Recipient private key is missing from the OS keychain.".to_string())?;
    let _ = recipient_private_key;

    let plaintext = elyra_crypto::open_message_envelope(
        &envelope,
        &recipient_key_id,
        &private_key,
        &sender_public_key,
    )
    .map_err(|error| format!("Could not open Elyra message envelope: {error}"))?;
    String::from_utf8(plaintext)
        .map_err(|_| "Decrypted Elyra envelope payload is not UTF-8.".to_string())
}

#[tauri::command]
pub fn fingerprint(value: String) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
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
    fn encrypts_and_decrypts_with_generated_key() {
        let key = generate_encryption_key().expect("key");
        let encrypted = encrypt_text(key.clone(), "private note".to_string()).expect("encrypt");
        assert_ne!(encrypted, "private note");
        assert_eq!(
            decrypt_text(key, encrypted).expect("decrypt"),
            "private note"
        );
    }

    #[test]
    fn rejects_wrong_key() {
        let first = generate_encryption_key().expect("key");
        let second = generate_encryption_key().expect("key");
        let encrypted = encrypt_text(first, "private note".to_string()).expect("encrypt");
        assert!(decrypt_text(second, encrypted).is_err());
    }

    fn trust_database() -> rusqlite::Connection {
        let database = rusqlite::Connection::open_in_memory().expect("in-memory database");
        database
            .execute(
                "CREATE TABLE crypto_contacts (
                    id INTEGER PRIMARY KEY,
                    trust_state TEXT NOT NULL
                )",
                [],
            )
            .expect("create trust table");
        database
    }

    fn insert_trust(database: &rusqlite::Connection, id: i64, state: &str) {
        database
            .execute(
                "INSERT INTO crypto_contacts (id, trust_state) VALUES (?1, ?2)",
                params![id, state],
            )
            .expect("insert trust state");
    }

    #[test]
    fn recipient_trust_gate_blocks_untrusted_states() {
        let database = trust_database();
        insert_trust(&database, 1, "verified");
        insert_trust(&database, 2, "unverified");
        insert_trust(&database, 3, "changed");
        insert_trust(&database, 4, "revoked");

        assert!(ensure_recipient_trust(&database, 1).is_ok());
        assert!(ensure_recipient_trust(&database, 2).is_err());
        assert!(ensure_recipient_trust(&database, 3).is_err());
        assert!(ensure_recipient_trust(&database, 4).is_err());
        assert!(ensure_recipient_trust(&database, 99).is_err());
    }
}
