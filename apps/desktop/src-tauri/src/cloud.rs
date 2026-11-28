//! Elyra Cloud bridge.
//!
//! Account sessions, notifications and sync cursors are handled by the Rust
//! side. The webview never receives the raw session after it has been stored;
//! it only receives the account profile needed to render the interface.

use std::time::Duration;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{secrets, storage};

const SESSION_KEY: &str = "cloud_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudUser {
    pub id: String,
    pub email: String,
    #[serde(alias = "display_name")]
    pub display_name: String,
    #[serde(alias = "created_at")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSession {
    pub access_token: String,
    pub expires_at: String,
    pub user: CloudUser,
}

/// The only session shape exposed to the webview. The access token stays in
/// the Rust-owned SQLite settings table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSessionView {
    pub expires_at: String,
    pub user: CloudUser,
    pub signed_in: bool,
}

impl From<&CloudSession> for CloudSessionView {
    fn from(session: &CloudSession) -> Self {
        Self {
            expires_at: session.expires_at.clone(),
            user: session.user.clone(),
            signed_in: !session_expired(session),
        }
    }
}

/// A token that has expired must not be presented as a usable session: every
/// protected cloud command would fail with 401 while the UI showed a sign-in.
fn session_expired(session: &CloudSession) -> bool {
    chrono::DateTime::parse_from_rfc3339(&session.expires_at)
        .map(|expires| expires.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
}

#[derive(Debug, Deserialize)]
struct SessionResponse {
    access_token: String,
    expires_at: String,
    user: CloudUser,
}

#[derive(Debug, Serialize)]
struct Credentials<'a> {
    email: &'a str,
    password: &'a str,
    display_name: &'a str,
}

#[derive(Debug, Serialize)]
struct NotificationInput<'a> {
    recipient_email: &'a str,
    title: &'a str,
    body: &'a str,
}

#[derive(Debug, Serialize)]
struct CursorInput<'a> {
    cursor: Option<&'a str>,
}

pub(crate) fn api_base(app: &AppHandle) -> Result<String, String> {
    if let Ok(value) = std::env::var("ELYRA_API_URL") {
        if !value.trim().is_empty() {
            return Ok(value.trim_end_matches('/').to_string());
        }
    }
    let database = storage::connection(app)?;
    let value: Option<String> = database
        .query_row(
            "SELECT value FROM settings WHERE key = 'api_url'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some(value) = value {
        let value = value.trim_end_matches('/').to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }
    if let Some(value) = option_env!("ELYRA_DEFAULT_API_URL") {
        if !value.trim().is_empty() {
            return Ok(value.trim_end_matches('/').to_string());
        }
    }
    Ok("http://127.0.0.1:8787".to_string())
}

#[tauri::command]
pub fn cloud_api_url(app: AppHandle) -> Result<String, String> {
    api_base(&app)
}

#[tauri::command]
pub fn cloud_set_api_url(app: AppHandle, url: String) -> Result<String, String> {
    let trimmed = url.trim().trim_end_matches('/').to_string();
    let database = storage::connection(&app)?;
    if trimmed.is_empty() {
        database
            .execute("DELETE FROM settings WHERE key = 'api_url'", [])
            .map_err(|error| error.to_string())?;
    } else {
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            return Err("Server URL must start with http:// or https://".to_string());
        }
        database
            .execute(
                "INSERT INTO settings (key, value) VALUES ('api_url', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![trimmed],
            )
            .map_err(|error| error.to_string())?;
    }
    api_base(&app)
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())
}

fn parse_response(response: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    let status = response.status();
    let text = response.text().map_err(|error| error.to_string())?;
    if !status.is_success() {
        let message = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| {
                value
                    .get("message")
                    .and_then(|message| message.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| format!("Elyra Cloud request failed ({status})"));
        return Err(message);
    }
    if text.trim().is_empty() {
        Ok(serde_json::Value::Null)
    } else {
        serde_json::from_str(&text).map_err(|error| error.to_string())
    }
}

fn save_session(app: &AppHandle, session: &CloudSession) -> Result<(), String> {
    let database = storage::connection(app)?;
    let raw = serde_json::to_string(session).map_err(|error| error.to_string())?;
    // The OS keychain is the primary store. If it is unavailable (for example
    // on a locked-down machine) keep the legacy SQLite row so the user can
    // still sign in; it is deleted again as soon as the keychain works.
    match secrets::set_secret(secrets::cloud_session_user(), &raw) {
        Ok(()) => {
            database
                .execute("DELETE FROM settings WHERE key = ?1", params![SESSION_KEY])
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(_) => database
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![SESSION_KEY, raw],
            )
            .map(|_| ())
            .map_err(|error| error.to_string()),
    }
}

fn discard_session(app: &AppHandle) -> Result<(), String> {
    let keychain_result = secrets::delete_secret(secrets::cloud_session_user());
    let database = storage::connection(app)?;
    database
        .execute("DELETE FROM settings WHERE key = ?1", params![SESSION_KEY])
        .map_err(|error| error.to_string())?;
    keychain_result
}

fn load_session(app: &AppHandle) -> Result<Option<CloudSession>, String> {
    if let Ok(Some(raw)) = secrets::get_secret(secrets::cloud_session_user()) {
        let session: CloudSession =
            serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        if session_expired(&session) {
            discard_session(app)?;
            return Ok(None);
        }
        return Ok(Some(session));
    }

    let database = storage::connection(app)?;
    let raw: Option<String> = database
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SESSION_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match raw {
        Some(raw) => {
            let session: CloudSession =
                serde_json::from_str(&raw).map_err(|error| error.to_string())?;
            if session_expired(&session) {
                discard_session(app)?;
                return Ok(None);
            }
            // Migrate sessions written by 0.2.0 into the OS keychain.
            if secrets::set_secret(secrets::cloud_session_user(), &raw).is_ok() {
                database
                    .execute("DELETE FROM settings WHERE key = ?1", params![SESSION_KEY])
                    .map_err(|error| error.to_string())?;
            }
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

fn require_session(app: &AppHandle) -> Result<CloudSession, String> {
    load_session(app)?.ok_or_else(|| "Sign in to Elyra Cloud first".to_string())
}

fn session_from_response(value: serde_json::Value) -> Result<CloudSession, String> {
    let response: SessionResponse =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    Ok(CloudSession {
        access_token: response.access_token,
        expires_at: response.expires_at,
        user: response.user,
    })
}

#[tauri::command(async)]
pub fn cloud_register(
    app: AppHandle,
    email: String,
    password: String,
    display_name: String,
) -> Result<CloudSessionView, String> {
    let url = format!("{}/v1/auth/register", api_base(&app)?);
    let response = client()?
        .post(url)
        .json(&Credentials {
            email: &email,
            password: &password,
            display_name: &display_name,
        })
        .send()
        .map_err(|error| error.to_string())?;
    let session = session_from_response(parse_response(response)?)?;
    save_session(&app, &session)?;
    Ok(CloudSessionView::from(&session))
}

#[tauri::command(async)]
pub fn cloud_login(
    app: AppHandle,
    email: String,
    password: String,
) -> Result<CloudSessionView, String> {
    let url = format!("{}/v1/auth/login", api_base(&app)?);
    let response = client()?
        .post(url)
        .json(&Credentials {
            email: &email,
            password: &password,
            display_name: "Elyra user",
        })
        .send()
        .map_err(|error| error.to_string())?;
    let session = session_from_response(parse_response(response)?)?;
    save_session(&app, &session)?;
    Ok(CloudSessionView::from(&session))
}

#[tauri::command(async)]
pub fn cloud_session(app: AppHandle) -> Result<Option<CloudSessionView>, String> {
    Ok(load_session(&app)?.as_ref().map(CloudSessionView::from))
}

#[tauri::command(async)]
pub fn cloud_logout(app: AppHandle) -> Result<(), String> {
    let keychain_result = secrets::delete_secret(secrets::cloud_session_user());
    let database = storage::connection(&app)?;
    database
        .execute("DELETE FROM settings WHERE key = ?1", params![SESSION_KEY])
        .map_err(|error| error.to_string())?;
    keychain_result
}

#[tauri::command(async)]
pub fn cloud_list_notifications(app: AppHandle) -> Result<serde_json::Value, String> {
    let session = require_session(&app)?;
    let url = format!("{}/v1/notifications", api_base(&app)?);
    let response = client()?
        .get(url)
        .bearer_auth(&session.access_token)
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)
}

#[tauri::command(async)]
pub fn cloud_send_notification(
    app: AppHandle,
    recipient_email: String,
    title: String,
    body: String,
) -> Result<serde_json::Value, String> {
    let session = require_session(&app)?;
    let url = format!("{}/v1/notifications", api_base(&app)?);
    let response = client()?
        .post(url)
        .bearer_auth(&session.access_token)
        .json(&NotificationInput {
            recipient_email: &recipient_email,
            title: &title,
            body: &body,
        })
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)
}

#[tauri::command(async)]
pub fn cloud_mark_notification_read(app: AppHandle, id: String) -> Result<(), String> {
    let session = require_session(&app)?;
    let url = format!("{}/v1/notifications/{}/read", api_base(&app)?, id);
    let response = client()?
        .patch(url)
        .bearer_auth(&session.access_token)
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response).map(|_| ())
}

#[tauri::command(async)]
pub fn cloud_sync_cursor(app: AppHandle) -> Result<serde_json::Value, String> {
    let session = require_session(&app)?;
    let url = format!("{}/v1/sync/cursor", api_base(&app)?);
    let response = client()?
        .get(url)
        .bearer_auth(&session.access_token)
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)
}

#[tauri::command(async)]
pub fn cloud_update_cursor(app: AppHandle, cursor: Option<String>) -> Result<(), String> {
    let session = require_session(&app)?;
    let url = format!("{}/v1/sync/cursor", api_base(&app)?);
    let response = client()?
        .put(url)
        .bearer_auth(&session.access_token)
        .json(&CursorInput {
            cursor: cursor.as_deref(),
        })
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response).map(|_| ())
}

#[tauri::command(async)]
pub fn cloud_health(app: AppHandle) -> Result<serde_json::Value, String> {
    let url = format!("{}/health", api_base(&app)?);
    let response = client()?
        .get(url)
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)
}

#[derive(Debug, Serialize)]
struct MailInput<'a> {
    to: &'a str,
    subject: &'a str,
    body: &'a str,
}

/// Send mail through the Elyra server relay. Returns the display name of the
/// signed-in sender so the local Sent folder can show who sent the message.
pub(crate) fn relay_send(
    app: &AppHandle,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    let session = require_session(app)?;
    let url = format!("{}/v1/mail/send", api_base(app)?);
    let response = client()?
        .post(url)
        .bearer_auth(&session.access_token)
        .json(&MailInput { to, subject, body })
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)?;
    Ok(session.user.display_name)
}

#[tauri::command(async)]
pub fn cloud_mail_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let session = require_session(&app)?;
    let url = format!("{}/v1/mail/status", api_base(&app)?);
    let response = client()?
        .get(url)
        .bearer_auth(&session.access_token)
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(expires_at: &str) -> CloudSession {
        CloudSession {
            access_token: "test-token".to_string(),
            expires_at: expires_at.to_string(),
            user: CloudUser {
                id: "id".to_string(),
                email: "user@example.test".to_string(),
                display_name: "User".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
        }
    }

    #[test]
    fn expired_and_malformed_sessions_are_not_signed_in() {
        assert!(session_expired(&session("2020-01-01T00:00:00Z")));
        assert!(session_expired(&session("not-a-date")));
        assert!(!CloudSessionView::from(&session("2020-01-01T00:00:00Z")).signed_in);

        let future = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        assert!(!session_expired(&session(&future)));
        assert!(CloudSessionView::from(&session(&future)).signed_in);
    }
}
