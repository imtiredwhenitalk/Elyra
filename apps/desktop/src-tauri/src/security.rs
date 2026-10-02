use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::AppHandle;

use crate::storage;

const AUDIT_RETENTION_LIMIT: i64 = 500;

#[derive(Debug, Serialize)]
pub struct SecurityStatus {
    pub status: String,
    pub checks_passed: i64,
    pub checks_total: i64,
    pub foreign_keys: bool,
    pub journal_mode: String,
    pub secure_delete: bool,
    pub audit_events: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub id: i64,
    pub created_at: String,
    pub event_type: String,
    pub outcome: String,
    pub details: String,
}

pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                event_type TEXT NOT NULL,
                outcome TEXT NOT NULL CHECK(outcome IN ('success', 'failure', 'warning')),
                details TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_audit_events_created_at ON audit_events(created_at DESC);",
        )
        .map_err(|error| error.to_string())?;
    record_event(&database, "security.initialized", "success", "Local security controls initialized")?;
    let _ = get_status(&database)?;
    Ok(())
}

fn check_boolean(database: &Connection, pragma: &str) -> Result<bool, String> {
    database
        .query_row(&format!("PRAGMA {pragma}"), [], |row| row.get::<_, i64>(0))
        .map(|value| value != 0)
        .map_err(|error| error.to_string())
}

fn get_status(database: &Connection) -> Result<SecurityStatus, String> {
    let foreign_keys = check_boolean(database, "foreign_keys")?;
    let secure_delete = check_boolean(database, "secure_delete")?;
    let journal_mode = database
        .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    let checks = [foreign_keys, secure_delete, journal_mode.eq_ignore_ascii_case("wal")];
    let checks_passed = checks.iter().filter(|check| **check).count() as i64;
    let audit_events = database
        .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;

    Ok(SecurityStatus {
        status: if checks_passed == checks.len() as i64 { "protected" } else { "attention" }.to_string(),
        checks_passed,
        checks_total: checks.len() as i64,
        foreign_keys,
        journal_mode,
        secure_delete,
        audit_events,
    })
}

fn record_event(database: &Connection, event_type: &str, outcome: &str, details: &str) -> Result<(), String> {
    database
        .execute(
            "INSERT INTO audit_events (event_type, outcome, details) VALUES (?1, ?2, ?3)",
            params![event_type, outcome, details],
        )
        .map_err(|error| error.to_string())?;
    database
        .execute(
            "DELETE FROM audit_events WHERE id NOT IN (
                SELECT id FROM audit_events ORDER BY id DESC LIMIT ?1
            )",
            params![AUDIT_RETENTION_LIMIT],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_security_status(app: AppHandle) -> Result<SecurityStatus, String> {
    let database = storage::connection(&app)?;
    let status = get_status(&database)?;
    let outcome = if status.status == "protected" { "success" } else { "warning" };
    record_event(&database, "security.check", outcome, "SQLite security policy check completed")?;
    Ok(status)
}

#[tauri::command]
pub fn list_audit_events(app: AppHandle, limit: Option<i64>) -> Result<Vec<AuditEvent>, String> {
    let database = storage::connection(&app)?;
    let bounded_limit = limit.unwrap_or(20).clamp(1, 100);
    let mut query = database
        .prepare(
            "SELECT id, created_at, event_type, outcome, details
             FROM audit_events ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = query
        .query_map(params![bounded_limit], |row| {
            Ok(AuditEvent {
                id: row.get(0)?,
                created_at: row.get(1)?,
                event_type: row.get(2)?,
                outcome: row.get(3)?,
                details: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}
