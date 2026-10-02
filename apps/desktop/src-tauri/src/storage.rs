use rusqlite::{params, Connection, Result as SqlResult};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
pub struct CachedEmail {
    pub id: i64,
    pub provider_id: String,
    pub sender: String,
    pub subject: String,
    pub body_text: String,
    pub folder: String,
    pub is_read: bool,
    pub is_starred: bool,
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("elyra.sqlite3"))
}

pub(crate) fn connection(app: &AppHandle) -> Result<Connection, String> {
    let path = database_path(app)?;
    let database = Connection::open(path).map_err(|error| error.to_string())?;
    database
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA secure_delete = ON;")
        .map_err(|error| error.to_string())?;
    Ok(database)
}

pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = connection(app)?;
    database
        .execute_batch(
            "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS accounts (
             id INTEGER PRIMARY KEY,
             provider TEXT NOT NULL,
             email TEXT NOT NULL UNIQUE,
             display_name TEXT NOT NULL,
             created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS folders (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL UNIQUE,
             kind TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS emails (
             id INTEGER PRIMARY KEY,
             account_id INTEGER REFERENCES accounts(id) ON DELETE CASCADE,
             provider_id TEXT NOT NULL,
             sender TEXT NOT NULL,
             subject TEXT NOT NULL,
             body_text TEXT NOT NULL DEFAULT '',
             body_html TEXT,
             folder_id INTEGER NOT NULL REFERENCES folders(id),
             is_read INTEGER NOT NULL DEFAULT 0,
             is_starred INTEGER NOT NULL DEFAULT 0,
             received_at TEXT NOT NULL,
             UNIQUE(account_id, provider_id)
         );
         CREATE TABLE IF NOT EXISTS contacts (
             id INTEGER PRIMARY KEY,
             email TEXT NOT NULL UNIQUE,
             display_name TEXT,
             avatar_url TEXT
         );
         CREATE TABLE IF NOT EXISTS settings (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );
         INSERT OR IGNORE INTO folders (name, kind) VALUES
             ('Inbox', 'system'), ('Starred', 'system'), ('Sent', 'system'),
             ('Drafts', 'system'), ('Snoozed', 'system'), ('Spam', 'system'),
             ('Trash', 'system'), ('Important', 'label'), ('Updates', 'label'),
             ('Security', 'label');
         INSERT OR IGNORE INTO settings (key, value) VALUES ('theme', 'light');",
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn initialize_local_storage(app: AppHandle) -> Result<(), String> {
    initialize(&app)
}

#[tauri::command]
pub fn cache_email(
    app: AppHandle,
    account_id: i64,
    provider_id: String,
    sender: String,
    subject: String,
    body_text: String,
    folder: String,
    received_at: String,
) -> Result<(), String> {
    let database = connection(&app)?;
    let folder_id: i64 = database
        .query_row(
            "SELECT id FROM folders WHERE name = ?1",
            params![folder],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    database
        .execute(
            "INSERT INTO emails (account_id, provider_id, sender, subject, body_text, folder_id, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(account_id, provider_id) DO UPDATE SET sender = excluded.sender,
             subject = excluded.subject, body_text = excluded.body_text, folder_id = excluded.folder_id,
             received_at = excluded.received_at",
            params![account_id, provider_id, sender, subject, body_text, folder_id, received_at],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_cached_emails(app: AppHandle) -> Result<Vec<CachedEmail>, String> {
    let database = connection(&app)?;
    let mut query = database
        .prepare(
            "SELECT emails.id, emails.provider_id, emails.sender, emails.subject, emails.body_text,
             folders.name, emails.is_read, emails.is_starred
             FROM emails JOIN folders ON folders.id = emails.folder_id ORDER BY emails.received_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = query
        .query_map([], |row| {
            Ok(CachedEmail {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                sender: row.get(2)?,
                subject: row.get(3)?,
                body_text: row.get(4)?,
                folder: row.get(5)?,
                is_read: row.get::<_, i64>(6)? != 0,
                is_starred: row.get::<_, i64>(7)? != 0,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<SqlResult<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_local_setting(app: AppHandle, key: String, value: String) -> Result<(), String> {
    let database = connection(&app)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}
