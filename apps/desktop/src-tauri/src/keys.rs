//! Cryptographic key lifecycle for the desktop client.
//!
//! Elyra generates Ed25519 identity keys locally. The public key and its
//! fingerprint live in SQLite; the private key is written only to the OS
//! keychain and is never returned to the webview or sent to a server. The
//! tables and commands are the foundation for end-to-end encrypted mail.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::AppHandle;

use crate::{crypto, secrets, security, storage};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoKeyView {
    pub id: i64,
    pub label: String,
    pub algorithm: String,
    pub public_key: String,
    pub fingerprint: String,
    pub status: String,
    pub created_at: String,
    pub rotated_at: Option<String>,
    pub revoked_at: Option<String>,
    pub has_private_key: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoContactView {
    pub id: i64,
    pub email: String,
    pub label: String,
    pub public_key: String,
    pub fingerprint: String,
    pub imported_at: String,
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS crypto_keys (
                id INTEGER PRIMARY KEY,
                label TEXT NOT NULL,
                algorithm TEXT NOT NULL DEFAULT 'Ed25519',
                public_key TEXT NOT NULL,
                fingerprint TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL DEFAULT 'active'
                    CHECK(status IN ('active', 'retired', 'revoked')),
                created_at TEXT NOT NULL,
                rotated_at TEXT,
                revoked_at TEXT
            );
            CREATE TABLE IF NOT EXISTS crypto_contacts (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                label TEXT NOT NULL DEFAULT '',
                public_key TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                imported_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_crypto_keys_status ON crypto_keys(status);",
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn has_private_key(id: i64) -> bool {
    secrets::get_secret(&secrets::crypto_key_user(id))
        .ok()
        .flatten()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn key_view(database: &Connection, id: i64) -> Result<CryptoKeyView, String> {
    database
        .query_row(
            "SELECT id, label, algorithm, public_key, fingerprint, status,
                    created_at, rotated_at, revoked_at
             FROM crypto_keys WHERE id = ?1",
            params![id],
            |row| {
                let key_id: i64 = row.get(0)?;
                Ok(CryptoKeyView {
                    id: key_id,
                    label: row.get(1)?,
                    algorithm: row.get(2)?,
                    public_key: row.get(3)?,
                    fingerprint: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    rotated_at: row.get(7)?,
                    revoked_at: row.get(8)?,
                    has_private_key: has_private_key(key_id),
                })
            },
        )
        .map_err(|error| error.to_string())
}

fn contact_view(database: &Connection, id: i64) -> Result<CryptoContactView, String> {
    database
        .query_row(
            "SELECT id, email, label, public_key, fingerprint, imported_at
             FROM crypto_contacts WHERE id = ?1",
            params![id],
            |row| {
                Ok(CryptoContactView {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    label: row.get(2)?,
                    public_key: row.get(3)?,
                    fingerprint: row.get(4)?,
                    imported_at: row.get(5)?,
                })
            },
        )
        .map_err(|error| error.to_string())
}

fn create_key(
    database: &Connection,
    label: &str,
) -> Result<CryptoKeyView, String> {
    let signing = SigningKey::generate(&mut OsRng);
    let public_bytes = signing.verifying_key().to_bytes();
    let secret_bytes = signing.to_bytes();
    let public_key = BASE64.encode(public_bytes);
    let fingerprint = crypto::fingerprint(public_key.clone());
    let now = now_string();

    database
        .execute(
            "INSERT INTO crypto_keys (label, algorithm, public_key, fingerprint, status, created_at)
             VALUES (?1, 'Ed25519', ?2, ?3, 'active', ?4)",
            params![label, public_key, fingerprint, now],
        )
        .map_err(|error| error.to_string())?;
    let id = database.last_insert_rowid();
    if let Err(error) = secrets::set_secret(
        &secrets::crypto_key_user(id),
        &BASE64.encode(secret_bytes),
    ) {
        let _ = database.execute("DELETE FROM crypto_keys WHERE id = ?1", params![id]);
        return Err(error);
    }
    security::record_event(
        database,
        "crypto.key_generate",
        "success",
        &format!("Generated Ed25519 identity key {id}"),
    )?;
    key_view(database, id)
}

#[tauri::command]
pub fn list_crypto_keys(app: AppHandle) -> Result<Vec<CryptoKeyView>, String> {
    let database = storage::connection(&app)?;
    let mut statement = database
        .prepare(
            "SELECT id FROM crypto_keys
             ORDER BY CASE status WHEN 'active' THEN 0 WHEN 'retired' THEN 1 ELSE 2 END,
                      created_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    ids.into_iter().map(|id| key_view(&database, id)).collect()
}

#[tauri::command]
pub fn generate_crypto_key(
    app: AppHandle,
    label: Option<String>,
) -> Result<CryptoKeyView, String> {
    let database = storage::connection(&app)?;
    let active: Option<i64> = database
        .query_row(
            "SELECT id FROM crypto_keys WHERE status = 'active' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if active.is_some() {
        return Err("An active identity key already exists. Rotate it instead.".to_string());
    }
    let label = label
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Primary identity".to_string());
    create_key(&database, &label)
}

#[tauri::command]
pub fn rotate_crypto_key(
    app: AppHandle,
    id: i64,
    label: Option<String>,
) -> Result<CryptoKeyView, String> {
    let database = storage::connection(&app)?;
    let existing: Option<(String, String)> = database
        .query_row(
            "SELECT label, status FROM crypto_keys WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((existing_label, status)) = existing else {
        return Err("Identity key not found.".to_string());
    };
    if status == "revoked" {
        return Err("A revoked key cannot be rotated. Generate a new identity key.".to_string());
    }
    let label = label
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or(existing_label);
    let created = create_key(&database, &label)?;
    let now = now_string();
    database
        .execute(
            "UPDATE crypto_keys SET status = 'retired', rotated_at = ?2 WHERE id = ?1",
            params![id, now],
        )
        .map_err(|error| error.to_string())?;
    let _ = secrets::delete_secret(&secrets::crypto_key_user(id));
    security::record_event(
        &database,
        "crypto.key_rotate",
        "success",
        &format!("Rotated identity key {id} to key {}", created.id),
    )?;
    Ok(created)
}

#[tauri::command]
pub fn revoke_crypto_key(app: AppHandle, id: i64) -> Result<CryptoKeyView, String> {
    let database = storage::connection(&app)?;
    let changed = database
        .execute(
            "UPDATE crypto_keys SET status = 'revoked', revoked_at = ?2 WHERE id = ?1",
            params![id, now_string()],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Identity key not found.".to_string());
    }
    let _ = secrets::delete_secret(&secrets::crypto_key_user(id));
    security::record_event(
        &database,
        "crypto.key_revoke",
        "warning",
        &format!("Revoked identity key {id} and deleted its private material"),
    )?;
    key_view(&database, id)
}

#[tauri::command]
pub fn export_public_key(app: AppHandle, id: i64) -> Result<String, String> {
    let database = storage::connection(&app)?;
    let public_key: String = database
        .query_row(
            "SELECT public_key FROM crypto_keys WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Identity key not found.".to_string())?;
    Ok(format!(
        "-----BEGIN ELYRA PUBLIC KEY-----\n{public_key}\n-----END ELYRA PUBLIC KEY-----"
    ))
}

fn normalize_public_key(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim_start().starts_with("-----"))
        .flat_map(|line| line.chars())
        .filter(|character| !character.is_whitespace())
        .collect()
}

#[tauri::command]
pub fn import_public_key(
    app: AppHandle,
    email: String,
    label: Option<String>,
    public_key: String,
) -> Result<CryptoContactView, String> {
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() < 5 {
        return Err("Enter the contact's email address.".to_string());
    }
    let encoded = normalize_public_key(&public_key);
    let bytes = BASE64
        .decode(&encoded)
        .map_err(|_| "The public key is not valid base64.".to_string())?;
    if bytes.len() != 32 {
        return Err("An Ed25519 public key must contain 32 bytes.".to_string());
    }
    let mut array = [0_u8; 32];
    array.copy_from_slice(&bytes);
    VerifyingKey::from_bytes(&array)
        .map_err(|_| "The public key is not a valid Ed25519 point.".to_string())?;
    let fingerprint = crypto::fingerprint(BASE64.encode(array));
    let label = label
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| email.clone());
    let database = storage::connection(&app)?;
    database
        .execute(
            "INSERT INTO crypto_contacts (email, label, public_key, fingerprint, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(email) DO UPDATE SET
                label = excluded.label,
                public_key = excluded.public_key,
                fingerprint = excluded.fingerprint,
                imported_at = excluded.imported_at",
            params![email, label, BASE64.encode(array), fingerprint, now_string()],
        )
        .map_err(|error| error.to_string())?;
    let id: i64 = database
        .query_row(
            "SELECT id FROM crypto_contacts WHERE email = ?1",
            params![email],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    security::record_event(
        &database,
        "crypto.contact_import",
        "success",
        &format!("Imported public key for {email}"),
    )?;
    contact_view(&database, id)
}

#[tauri::command]
pub fn list_crypto_contacts(app: AppHandle) -> Result<Vec<CryptoContactView>, String> {
    let database = storage::connection(&app)?;
    let mut statement = database
        .prepare(
            "SELECT id FROM crypto_contacts ORDER BY lower(label), lower(email)",
        )
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    ids.into_iter()
        .map(|id| contact_view(&database, id))
        .collect()
}

#[tauri::command]
pub fn delete_crypto_contact(app: AppHandle, id: i64) -> Result<(), String> {
    let database = storage::connection(&app)?;
    let changed = database
        .execute("DELETE FROM crypto_contacts WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Contact key not found.".to_string());
    }
    security::record_event(
        &database,
        "crypto.contact_delete",
        "success",
        &format!("Removed contact key {id}"),
    )?;
    Ok(())
}
