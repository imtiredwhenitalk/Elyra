//! Offline mailbox state and the durable outgoing queue.
//!
//! Messages that cannot be sent are kept in SQLite until connectivity returns.
//! The queue never sends anything in the background on its own: the user can
//! flush it from Settings, and the desktop UI flushes it when the browser
//! reports that the machine is back online.

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::AppHandle;

use crate::{reporting, sendemail, storage};

const MAX_ATTEMPTS: i64 = 5;

#[derive(Debug, Clone, Serialize)]
pub struct OutgoingMessage {
    pub id: i64,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub account_id: Option<i64>,
    pub created_at: String,
    pub attempts: i64,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueFlushResult {
    pub sent: i64,
    pub failed: i64,
    pub blocked: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub offline_mode: bool,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub queued: i64,
    pub failed: i64,
    pub blocked: i64,
    pub accounts: i64,
    pub updated_at: String,
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn setting(database: &rusqlite::Connection, key: &str) -> Option<String> {
    database
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS outgoing_queue (
                 id INTEGER PRIMARY KEY,
                 recipient TEXT NOT NULL,
                 subject TEXT NOT NULL,
                 body TEXT NOT NULL,
                 account_id INTEGER,
                 created_at TEXT NOT NULL,
                 attempts INTEGER NOT NULL DEFAULT 0,
                 status TEXT NOT NULL DEFAULT 'queued',
                 last_error TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_outgoing_queue_status
                 ON outgoing_queue(status, id);
             INSERT OR IGNORE INTO settings (key, value) VALUES
                 ('sync_offline_mode', 'false');",
        )
        .map_err(|error| error.to_string())
}

pub(crate) fn mark_sync_error(app: &AppHandle, error: &str) {
    if let Ok(database) = storage::connection(app) {
        let _ = database.execute(
            "INSERT INTO settings (key, value) VALUES ('sync_last_error', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![reporting::redact(error)],
        );
    }
}

#[tauri::command]
pub fn queue_outgoing_message(
    app: AppHandle,
    to: String,
    subject: String,
    body: String,
    account_id: Option<i64>,
) -> Result<i64, String> {
    let _recipient: lettre::message::Mailbox = to
        .trim()
        .parse()
        .map_err(|_| "Invalid recipient email address".to_string())?;
    let database = storage::connection(&app)?;
    database
        .execute(
            "INSERT INTO outgoing_queue (recipient, subject, body, account_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![to.trim(), subject, body, account_id, now_string()],
        )
        .map_err(|error| error.to_string())?;
    let id = database.last_insert_rowid();
    let _ = reporting::record(
        &app,
        "info",
        "sync",
        "outbox.queued",
        "An outgoing message was queued for later delivery",
    );
    Ok(id)
}

#[tauri::command]
pub fn list_outgoing_queue(app: AppHandle) -> Result<Vec<OutgoingMessage>, String> {
    let database = storage::connection(&app)?;
    let mut statement = database
        .prepare(
            "SELECT id, recipient, subject, body, account_id, created_at,
                    attempts, status, last_error
             FROM outgoing_queue ORDER BY created_at DESC, id DESC LIMIT 100",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(OutgoingMessage {
                id: row.get(0)?,
                recipient: row.get(1)?,
                subject: row.get(2)?,
                body: row.get(3)?,
                account_id: row.get(4)?,
                created_at: row.get(5)?,
                attempts: row.get(6)?,
                status: row.get(7)?,
                last_error: row.get(8)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command(async)]
pub fn flush_outgoing_queue(app: AppHandle) -> Result<QueueFlushResult, String> {
    let database = storage::connection(&app)?;
    let offline = setting(&database, "sync_offline_mode")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if offline {
        let queued: i64 = database
            .query_row(
                "SELECT COUNT(*) FROM outgoing_queue WHERE status IN ('queued', 'failed')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        return Ok(QueueFlushResult {
            sent: 0,
            failed: 0,
            blocked: 0,
            skipped: queued,
            errors: vec!["Offline mode is enabled; nothing was sent".to_string()],
        });
    }

    let pending: Vec<(i64, String, String, String, Option<i64>, i64)> = {
        let mut statement = database
            .prepare(
                "SELECT id, recipient, subject, body, account_id, attempts
                 FROM outgoing_queue
                 WHERE status IN ('queued', 'failed') AND attempts < ?1
                 ORDER BY id ASC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![MAX_ATTEMPTS], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };

    let mut result = QueueFlushResult {
        sent: 0,
        failed: 0,
        blocked: 0,
        skipped: 0,
        errors: Vec::new(),
    };

    for (id, recipient, subject, body, account_id, attempts) in pending {
        match sendemail::send_email(
            app.clone(),
            recipient.clone(),
            subject.clone(),
            body.clone(),
            account_id,
        ) {
            Ok(()) => {
                database
                    .execute("DELETE FROM outgoing_queue WHERE id = ?1", params![id])
                    .map_err(|error| error.to_string())?;
                result.sent += 1;
            }
            Err(error) => {
                let safe = reporting::redact(&error);
                let next_attempts = attempts + 1;
                let status = if next_attempts >= MAX_ATTEMPTS {
                    "blocked"
                } else {
                    "failed"
                };
                database
                    .execute(
                        "UPDATE outgoing_queue
                         SET attempts = ?2, status = ?3, last_error = ?4 WHERE id = ?1",
                        params![id, next_attempts, status, safe],
                    )
                    .map_err(|database_error| database_error.to_string())?;
                if status == "blocked" {
                    result.blocked += 1;
                } else {
                    result.failed += 1;
                }
                result.errors.push(safe.clone());
                mark_sync_error(&app, &safe);
                let _ = reporting::record(&app, "warning", "sync", "outbox.retry", &safe);
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn retry_outgoing_message(app: AppHandle, id: i64) -> Result<(), String> {
    let database = storage::connection(&app)?;
    let updated = database
        .execute(
            "UPDATE outgoing_queue
             SET status = 'queued', attempts = 0, last_error = NULL WHERE id = ?1",
            params![id],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err("Message is no longer in the outbox".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn discard_outgoing_message(app: AppHandle, id: i64) -> Result<(), String> {
    let database = storage::connection(&app)?;
    database
        .execute("DELETE FROM outgoing_queue WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_sync_status(app: AppHandle) -> Result<SyncStatus, String> {
    let database = storage::connection(&app)?;
    let count = |status: &str| -> i64 {
        database
            .query_row(
                "SELECT COUNT(*) FROM outgoing_queue WHERE status = ?1",
                params![status],
                |row| row.get(0),
            )
            .unwrap_or(0)
    };
    Ok(SyncStatus {
        offline_mode: setting(&database, "sync_offline_mode")
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        last_sync_at: setting(&database, "last_sync_at"),
        last_error: setting(&database, "sync_last_error"),
        queued: count("queued"),
        failed: count("failed"),
        blocked: count("blocked"),
        accounts: database
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap_or(0),
        updated_at: now_string(),
    })
}

#[tauri::command]
pub fn set_offline_mode(app: AppHandle, enabled: bool) -> Result<SyncStatus, String> {
    let database = storage::connection(&app)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES ('sync_offline_mode', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![if enabled { "true" } else { "false" }],
        )
        .map_err(|error| error.to_string())?;
    get_sync_status(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_only_accepts_parseable_recipients() {
        // Validation happens in the command via lettre; this test documents the
        // contract without opening a database.
        assert!("ada@example.com"
            .parse::<lettre::message::Mailbox>()
            .is_ok());
        assert!("not-an-address"
            .parse::<lettre::message::Mailbox>()
            .is_err());
    }
}
