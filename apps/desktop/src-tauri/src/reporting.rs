//! Privacy-preserving diagnostics for the desktop build.
//!
//! Events are stored locally in SQLite, are redacted before they are written,
//! and are only shared when the user exports a report. The optional crash
//! reporting preference is recorded here, but Elyra never uploads telemetry on
//! its own: there is no collection endpoint configured in this release.

use chrono::Utc;
use regex::Regex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{diagnostics, storage};

const MAX_EVENTS: i64 = 500;
const RETENTION_DAYS: i64 = 30;
const MAX_MESSAGE_CHARS: usize = 600;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticEvent {
    pub id: i64,
    pub created_at: String,
    pub level: String,
    pub category: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiagnosticEventInput {
    pub message: String,
    pub level: Option<String>,
    pub category: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReport {
    pub format: &'static str,
    pub app_version: &'static str,
    pub generated_at: String,
    pub note: &'static str,
    pub crash_reporting_enabled: bool,
    pub diagnostic_logging_enabled: bool,
    pub diagnostics: serde_json::Value,
    pub events: Vec<DiagnosticEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrashReportingStatus {
    pub enabled: bool,
    pub automatic_upload: bool,
    pub note: &'static str,
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Remove credentials, addresses and opaque identifiers before a message is
/// persisted. Redaction is intentionally aggressive: a diagnostic log is most
/// useful when it can be shared without a second privacy review.
pub fn redact(input: &str) -> String {
    let mut value = input.to_string();

    if let Ok(pattern) = Regex::new(
        r"(?i)\b(api[_-]?key|apikey|password|passphrase|secret|token|authorization|private[_-]?key)\b\s*[:=]\s*\S+",
    ) {
        value = pattern.replace_all(&value, "$1=[redacted]").to_string();
    }
    if let Ok(pattern) = Regex::new(r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{16,}") {
        value = pattern.replace_all(&value, "Bearer [redacted]").to_string();
    }
    if let Ok(pattern) =
        Regex::new(r"(?i)\b(?:sk|pk|rk|ghp|gho|glpat|xox[baprs]|ya29)[-_][A-Za-z0-9_-]{12,}")
    {
        value = pattern.replace_all(&value, "[redacted]").to_string();
    }
    if let Ok(pattern) = Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}") {
        value = pattern.replace_all(&value, "[email]").to_string();
    }
    if let Ok(pattern) = Regex::new(r"(https?://[^\s?#]+)[?#][^\s]*") {
        value = pattern.replace_all(&value, "$1").to_string();
    }
    if let Ok(pattern) = Regex::new(r"\b[A-Fa-f0-9]{32,}\b") {
        value = pattern.replace_all(&value, "[redacted]").to_string();
    }
    if let Ok(pattern) = Regex::new(r"\b[A-Za-z0-9+/]{40,}={0,2}\b") {
        value = pattern.replace_all(&value, "[redacted]").to_string();
    }

    value.trim().chars().take(MAX_MESSAGE_CHARS).collect()
}

fn prune(database: &Connection) -> Result<(), String> {
    database
        .execute_batch(&format!(
            "DELETE FROM diagnostic_events
                  WHERE created_at < datetime('now', '-{RETENTION_DAYS} days');
              DELETE FROM diagnostic_events WHERE id NOT IN (
                  SELECT id FROM diagnostic_events
                  ORDER BY created_at DESC, id DESC LIMIT {MAX_EVENTS}
              );"
        ))
        .map_err(|error| error.to_string())
}

fn setting_enabled(database: &Connection, key: &str, default: bool) -> bool {
    database
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .map(|value| !value.eq_ignore_ascii_case("false"))
        .unwrap_or(default)
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS diagnostic_events (
                 id INTEGER PRIMARY KEY,
                 created_at TEXT NOT NULL,
                 level TEXT NOT NULL,
                 category TEXT NOT NULL,
                 code TEXT NOT NULL,
                 message TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_diagnostic_events_created
                 ON diagnostic_events(created_at DESC);",
        )
        .map_err(|error| error.to_string())?;
    database
        .execute_batch(
            "INSERT OR IGNORE INTO settings (key, value) VALUES
                 ('diagnostic_logging_enabled', 'true'),
                 ('crash_reporting_enabled', 'false');",
        )
        .map_err(|error| error.to_string())?;
    prune(&database)
}

pub(crate) fn record(
    app: &AppHandle,
    level: &str,
    category: &str,
    code: &str,
    message: &str,
) -> Result<(), String> {
    let database = storage::connection(app)?;
    if !setting_enabled(&database, "diagnostic_logging_enabled", true) {
        return Ok(());
    }
    let normalized_level = match level.to_ascii_lowercase().as_str() {
        "warn" | "warning" => "warning",
        "error" | "fatal" => "error",
        _ => "info",
    };
    database
        .execute(
            "INSERT INTO diagnostic_events (created_at, level, category, code, message)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                now_string(),
                normalized_level,
                redact(category),
                redact(code),
                redact(message),
            ],
        )
        .map_err(|error| error.to_string())?;
    prune(&database)
}

#[tauri::command]
pub fn log_diagnostic_event(app: AppHandle, input: DiagnosticEventInput) -> Result<(), String> {
    record(
        &app,
        input.level.as_deref().unwrap_or("error"),
        input.category.as_deref().unwrap_or("app"),
        input.code.as_deref().unwrap_or("ui.exception"),
        &input.message,
    )
}

#[tauri::command]
pub fn list_diagnostic_events(
    app: AppHandle,
    limit: Option<i64>,
) -> Result<Vec<DiagnosticEvent>, String> {
    let database = storage::connection(&app)?;
    let limit = limit.unwrap_or(50).clamp(1, MAX_EVENTS);
    let mut statement = database
        .prepare(
            "SELECT id, created_at, level, category, code, message
             FROM diagnostic_events ORDER BY created_at DESC, id DESC LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![limit], |row| {
            Ok(DiagnosticEvent {
                id: row.get(0)?,
                created_at: row.get(1)?,
                level: row.get(2)?,
                category: row.get(3)?,
                code: row.get(4)?,
                message: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn export_diagnostic_report(app: AppHandle) -> Result<DiagnosticReport, String> {
    let events = list_diagnostic_events(app.clone(), Some(MAX_EVENTS))?;
    let enabled = crash_reporting_status(app.clone())?.enabled;
    let logging_enabled = diagnostic_logging_status(&app)?.enabled;
    let diagnostics = serde_json::to_value(diagnostics::get_app_diagnostics(app)?)
        .map_err(|error| error.to_string())?;
    Ok(DiagnosticReport {
        format: "elyra-diagnostic-report",
        app_version: env!("CARGO_PKG_VERSION"),
        generated_at: now_string(),
        note: "Elyra does not upload telemetry. Share this file only with someone you trust; it contains local counts and redacted error messages.",
        crash_reporting_enabled: enabled,
        diagnostic_logging_enabled: logging_enabled,
        diagnostics,
        events,
    })
}

fn crash_reporting_status(app: AppHandle) -> Result<CrashReportingStatus, String> {
    let database = storage::connection(&app)?;
    Ok(CrashReportingStatus {
        enabled: setting_enabled(&database, "crash_reporting_enabled", false),
        automatic_upload: false,
        note: "Optional preference for future crash reporting. No diagnostic data leaves this device automatically in 0.5.0.",
    })
}

#[tauri::command]
pub fn get_crash_reporting_status(app: AppHandle) -> Result<CrashReportingStatus, String> {
    crash_reporting_status(app)
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticLoggingStatus {
    pub enabled: bool,
}

fn diagnostic_logging_status(app: &AppHandle) -> Result<DiagnosticLoggingStatus, String> {
    let database = storage::connection(app)?;
    Ok(DiagnosticLoggingStatus {
        enabled: setting_enabled(&database, "diagnostic_logging_enabled", true),
    })
}

#[tauri::command]
pub fn get_diagnostic_logging_status(app: AppHandle) -> Result<DiagnosticLoggingStatus, String> {
    diagnostic_logging_status(&app)
}

#[tauri::command]
pub fn set_diagnostic_logging_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<DiagnosticLoggingStatus, String> {
    let database = storage::connection(&app)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES ('diagnostic_logging_enabled', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![if enabled { "true" } else { "false" }],
        )
        .map_err(|error| error.to_string())?;
    diagnostic_logging_status(&app)
}

#[tauri::command]
pub fn set_crash_reporting_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<CrashReportingStatus, String> {
    let database = storage::connection(&app)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES ('crash_reporting_enabled', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![if enabled { "true" } else { "false" }],
        )
        .map_err(|error| error.to_string())?;
    crash_reporting_status(app)
}

#[tauri::command]
pub fn clear_diagnostic_log(app: AppHandle) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let removed = database
        .execute("DELETE FROM diagnostic_events", [])
        .map_err(|error| error.to_string())?;
    Ok(removed as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_removes_secrets_and_addresses() {
        let message = "Failed for ada@example.com with api_key=abc123XYZ and Bearer sk-live-12345678901234567890";
        let redacted = redact(message);
        assert!(!redacted.contains("ada@example.com"));
        assert!(!redacted.contains("abc123XYZ"));
        assert!(!redacted.contains("sk-live"));
        assert!(redacted.contains("[email]"));
        assert!(redacted.contains("[redacted]"));
    }

    #[test]
    fn redaction_strips_url_query() {
        let redacted = redact("opened https://example.com/login?token=secret-value");
        assert!(redacted.contains("https://example.com/login"));
        assert!(!redacted.contains("token=secret-value"));
    }

    #[test]
    fn redaction_limits_length() {
        let redacted = redact(&"x".repeat(2000));
        assert_eq!(redacted.chars().count(), MAX_MESSAGE_CHARS);
    }
}
