//! Centralized management for optional third-party integrations.
//!
//! API keys are stored in the OS keychain under `integration-<provider>` and are
//! never returned to the webview. The SQLite table only records whether an
//! integration is enabled and the result of the last validation.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::AppHandle;

use crate::{secrets, security, storage, threats};

pub(crate) const VIRUSTOTAL: &str = "virustotal";
pub(crate) const GOOGLE_SAFE_BROWSING: &str = "google_safe_browsing";

struct IntegrationPreset {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    docs_url: &'static str,
    key_hint: &'static str,
}

const INTEGRATIONS: [IntegrationPreset; 2] = [
    IntegrationPreset {
        id: VIRUSTOTAL,
        label: "VirusTotal",
        description: "Checks links against 90+ security vendors and their URL reputation data.",
        docs_url: "https://www.virustotal.com/gui/my-apikey",
        key_hint: "Paste the API key from your VirusTotal profile.",
    },
    IntegrationPreset {
        id: GOOGLE_SAFE_BROWSING,
        label: "Google Safe Browsing",
        description: "Checks links against Google's constantly updated list of unsafe web resources.",
        docs_url: "https://developers.google.com/safe-browsing/v4/get-started",
        key_hint: "Paste a Google Cloud API key with the Safe Browsing API enabled.",
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationView {
    pub provider: String,
    pub label: String,
    pub description: String,
    pub docs_url: String,
    pub key_hint: String,
    pub enabled: bool,
    pub configured: bool,
    pub status: String,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
}

fn preset_for(provider: &str) -> Result<&'static IntegrationPreset, String> {
    INTEGRATIONS
        .iter()
        .find(|preset| preset.id == provider)
        .ok_or_else(|| "Unknown integration".to_string())
}

fn now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub(crate) fn provider_label(provider: &str) -> &'static str {
    INTEGRATIONS
        .iter()
        .find(|preset| preset.id == provider)
        .map(|preset| preset.label)
        .unwrap_or("Unknown provider")
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS api_integrations (
                provider TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'not_configured',
                last_checked_at TEXT,
                last_error TEXT,
                updated_at TEXT NOT NULL DEFAULT ''
            );",
        )
        .map_err(|error| error.to_string())?;
    for preset in INTEGRATIONS.iter() {
        database
            .execute(
                "INSERT OR IGNORE INTO api_integrations (provider) VALUES (?1)",
                params![preset.id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn configured(provider: &str) -> bool {
    secrets::get_secret(&secrets::integration_user(provider))
        .ok()
        .flatten()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn view_for(app: &AppHandle, preset: &IntegrationPreset) -> Result<IntegrationView, String> {
    let database = storage::connection(app)?;
    let row = database
        .query_row(
            "SELECT enabled, status, last_checked_at, last_error
             FROM api_integrations WHERE provider = ?1",
            params![preset.id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (enabled, stored_status, last_checked_at, last_error) =
        row.unwrap_or((false, "not_configured".to_string(), None, None));
    let is_configured = configured(preset.id);
    let status = if !is_configured {
        "not_configured".to_string()
    } else if stored_status == "not_configured" {
        "unverified".to_string()
    } else {
        stored_status
    };
    Ok(IntegrationView {
        provider: preset.id.to_string(),
        label: preset.label.to_string(),
        description: preset.description.to_string(),
        docs_url: preset.docs_url.to_string(),
        key_hint: preset.key_hint.to_string(),
        enabled: is_configured && enabled,
        configured: is_configured,
        status,
        last_checked_at,
        last_error,
    })
}

fn update_state(
    database: &Connection,
    provider: &str,
    enabled: bool,
    status: &str,
    last_error: Option<&str>,
) -> Result<(), String> {
    database
        .execute(
            "INSERT INTO api_integrations
                (provider, enabled, status, last_checked_at, last_error, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?4)
             ON CONFLICT(provider) DO UPDATE SET
                enabled = excluded.enabled,
                status = excluded.status,
                last_checked_at = excluded.last_checked_at,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at",
            params![
                provider,
                if enabled { 1 } else { 0 },
                status,
                now(),
                last_error
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_integrations(app: AppHandle) -> Result<Vec<IntegrationView>, String> {
    INTEGRATIONS
        .iter()
        .map(|preset| view_for(&app, preset))
        .collect()
}

#[tauri::command(async)]
pub fn connect_integration(
    app: AppHandle,
    provider: String,
    key: String,
) -> Result<IntegrationView, String> {
    let preset = preset_for(&provider)?;
    let key = key.trim();
    if key.len() < 8 {
        return Err(format!(
            "Enter the {} API key before connecting the integration.",
            preset.label
        ));
    }
    secrets::set_secret(&secrets::integration_user(preset.id), key)?;
    let database = storage::connection(&app)?;
    match threats::validate_provider(preset.id, key) {
        Ok(detail) => {
            update_state(&database, preset.id, true, "connected", None)?;
            security::record_event(
                &database,
                "integration.connect",
                "success",
                &format!("{} connected: {detail}", preset.label),
            )?;
        }
        Err(error) => {
            update_state(&database, preset.id, false, "error", Some(&error))?;
            security::record_event(
                &database,
                "integration.connect",
                "failure",
                &format!("{} validation failed", preset.label),
            )?;
        }
    }
    view_for(&app, preset)
}

#[tauri::command(async)]
pub fn test_integration(app: AppHandle, provider: String) -> Result<IntegrationView, String> {
    let preset = preset_for(&provider)?;
    let key = secrets::get_secret(&secrets::integration_user(preset.id))?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Add a {} API key first.", preset.label))?;
    let database = storage::connection(&app)?;
    match threats::validate_provider(preset.id, &key) {
        Ok(detail) => {
            update_state(&database, preset.id, true, "connected", None)?;
            security::record_event(
                &database,
                "integration.test",
                "success",
                &format!("{} accepted the API key: {detail}", preset.label),
            )?;
        }
        Err(error) => {
            update_state(&database, preset.id, false, "error", Some(&error))?;
            security::record_event(
                &database,
                "integration.test",
                "failure",
                &format!("{} rejected the API key", preset.label),
            )?;
        }
    }
    view_for(&app, preset)
}

#[tauri::command(async)]
pub fn set_integration_enabled(
    app: AppHandle,
    provider: String,
    enabled: bool,
) -> Result<IntegrationView, String> {
    let preset = preset_for(&provider)?;
    if !enabled {
        let database = storage::connection(&app)?;
        update_state(
            &database,
            preset.id,
            false,
            if configured(preset.id) {
                "disabled"
            } else {
                "not_configured"
            },
            None,
        )?;
        return view_for(&app, preset);
    }

    let key = secrets::get_secret(&secrets::integration_user(preset.id))?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Add a {} API key first.", preset.label))?;
    let database = storage::connection(&app)?;
    match threats::validate_provider(preset.id, &key) {
        Ok(detail) => {
            update_state(&database, preset.id, true, "connected", None)?;
            security::record_event(
                &database,
                "integration.enable",
                "success",
                &format!("{} enabled: {detail}", preset.label),
            )?;
            view_for(&app, preset)
        }
        Err(error) => {
            update_state(&database, preset.id, false, "error", Some(&error))?;
            Err(error)
        }
    }
}

#[tauri::command]
pub fn remove_integration_key(app: AppHandle, provider: String) -> Result<IntegrationView, String> {
    let preset = preset_for(&provider)?;
    secrets::delete_secret(&secrets::integration_user(preset.id))?;
    let database = storage::connection(&app)?;
    database
        .execute(
            "UPDATE api_integrations
             SET enabled = 0, status = 'not_configured', last_checked_at = NULL,
                 last_error = NULL, updated_at = ?2
             WHERE provider = ?1",
            params![preset.id, now()],
        )
        .map_err(|error| error.to_string())?;
    security::record_event(
        &database,
        "integration.remove",
        "success",
        &format!("{} disconnected", preset.label),
    )?;
    view_for(&app, preset)
}

pub(crate) fn active_providers(app: &AppHandle) -> Result<Vec<(String, String)>, String> {
    let database = storage::connection(app)?;
    let mut providers = Vec::new();
    for preset in INTEGRATIONS.iter() {
        let enabled: Option<i64> = database
            .query_row(
                "SELECT enabled FROM api_integrations WHERE provider = ?1",
                params![preset.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if enabled.unwrap_or(0) == 0 {
            continue;
        }
        if let Some(key) = secrets::get_secret(&secrets::integration_user(preset.id))
            .ok()
            .flatten()
            .filter(|value| !value.trim().is_empty())
        {
            providers.push((preset.id.to_string(), key));
        }
    }
    Ok(providers)
}
